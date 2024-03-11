import socket
import sys
import random
import time

def insert_kv(host, port, key_count, value_size):
    # create a socket object
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)

    # connect to the server
    s.connect((host, port))

    for i in range(key_count):
        key = f"key_{i}"
        value = str(random.randint(0, 10**value_size))
        s.sendall(f"*3\r\n$3\r\nSET\r\n${len(key)}\r\n{key}\r\n${len(value)}\r\n{value}\r\n".encode())
        data = s.recv(1024)
        print(f"Received: {data.decode()}")
    
    s.close()

if __name__ == "__main__":
    host = sys.argv[1]
    port = int(sys.argv[2])
    key_count = int(sys.argv[3])
    value_size = int(sys.argv[4])
    t = time.process_time()
    insert_kv(host, port, key_count, value_size)
    elapsed_time = time.process_time() - t
    
    print(f"Elapsed time: {elapsed_time} seconds")