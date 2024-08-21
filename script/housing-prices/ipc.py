import requests_unixsocket
import json
import logging

# Setup logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# NotReady = 1,
# DataMissing = 2,
# InComingFinish = 3,
# AlreadyFinish = 4,
# NoAvaiableData = 5,
class IPCError(Exception):
    def __init__(self, code: int, msg: str):
        self.code = code
        self.msg = msg
        super().__init__(f"Error {code}: {msg}")

class IPCClient:
    def __init__(self):
        #self.base_url = f'http+unix://%2Fhom e%2Fhunjixin%2Fcode%2Fjz-flow%2Ftest.d' unix_socket/compute_unit_runner_d
        self.base_url = f'http+unix://%2Funix_socket%2Fcompute_unit_runner_d'
        # Initialize the session
        self.session = requests_unixsocket.Session()

    def _send_request(self, method: str, endpoint: str, json_data: dict = None):
        url = f"{self.base_url}{endpoint}"
        headers = {"Content-Type": "application/json"}
        response = self.session.request(method, url, headers=headers, json=json_data)

        if response.status_code != 200:
            try:
                error_data = response.json()
                code = error_data.get("code", 0)
                message = error_data.get("msg", response.text)
                raise IPCError(code, message)
            except ValueError:
                raise IPCError(0, response.text)
            
        return response

    def finish(self) -> None:
        self._send_request("POST", "/api/v1/status")

    def status(self) -> dict:
        response = self._send_request("GET", "/api/v1/status")
        return response.json()

    def submit_output(self, req: dict) -> None:
        self._send_request("POST", "/api/v1/submit", req)

    def request_available_data(self, id: str = None) -> dict:
        endpoint = "/api/v1/data"
        if id:
            endpoint += f"?id={id}"
        response = self._send_request("GET", endpoint)
        return response.json()

    def complete_result(self, id: str) -> None:
        req = {"id": id}
        self._send_request("POST", "/api/v1/data", req)

# Example usage
if __name__ == "__main__":
    client = IPCClient()

    try:
        client.finish()
        status = client.status()
        print("Status:", status)

        available_data = client.request_available_data()
        print("Available Data:", available_data)

        client.complete_result('example-id')
        client.submit_output({"new_id": "example-id", "timeout": 30, "flags": 0, "count": 0})

    except IPCError as e:
        logger.error(f"IPC Error: {e}")
