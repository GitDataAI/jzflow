from jzflowsdk import simple_loop
import os
import subprocess
import sys

def count_files_in_directory(directory_path):
    file_count = 0
    for root, dirs, files in os.walk(directory_path):
        file_count += len(files)
    return file_count

def run_shell(root_input_dir, output):
    os.environ["INPUT"] = root_input_dir
    os.environ["OUTPUT"] = output
    
    print(sys.argv[1:])
    process = subprocess.Popen(sys.argv[1:])
    process.wait()
    
    filecount = count_files_in_directory(output)
    # Submit directory after completing a batch
    data = {
        "id": id,
        "size": filecount,
        "data_flag": {
            "is_keep_data": False,
            "is_transparent_data": False
        },
        "priority": 0,
    }
    return data

if __name__ == "__main__":
    simple_loop(run_shell)