import os
import time
from os import path
import joblib
import uuid
from ipc import IPCClient, IPCError
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

def simple_loop(callback):
    client = IPCClient()
    tmp_path = "/app/tmp"
    
    while True:
        instant = time.time()
        try:
            available_data = client.request_available_data()
            if available_data is not None:
                id = available_data['id']
                path_str = os.path.join(tmp_path, id)
                root_input_dir = path_str

                new_id = str(uuid.uuid4())
                output_dir = os.path.join(tmp_path, new_id)
                os.makedirs(output_dir, exist_ok=True)
                
                data = callback(root_input_dir, output_dir)
                
                logger.info("process data %s", time.time() - instant)

                client.complete_result(id)
                # Submit directory after completing a batch
                if data is not None:
                    if isinstance(data, list):
                        for item in data:
                            data.id = new_id
                            client.submit_output(item)
                    else:
                        data.id = new_id
                        client.submit_output(data)
                logger.info("submit new data %s", time.time() - instant)
            else:
                time.sleep(2)
                continue

        except IPCError as e:
            if e.code == 1:
                time.sleep(2)
                continue 
            elif e.code == 3: 
                client.finish()
                logger.info(f"incoming data finish {e.msg}")
                return 
            elif e.code == 4:
                logger.info(f"receive AlreadyFinish {e.msg}")
                time.sleep(60*60*24*365) # alway wait here to provent container restart
                return   
            elif e.code == 5:        
                logger.info(f"no avaiable data {e.msg}")
                time.sleep(2)
                continue
            else:
                logger.error("got unknown error %s", e)
                time.sleep(5)