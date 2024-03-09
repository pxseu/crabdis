#!/usr/bin/env python3

import socket
import threading
import datetime

# Configuration: Set the proxy's listening host, port, and the target server's host and port.
LISTEN_HOST = '0.0.0.0'  # Listen on all network interfaces
LISTEN_PORT = 9999       # Port to listen on
TARGET_HOST = 'localhost'  # Target server's hostname or IP address
TARGET_PORT = 6379        # Target server's port (default Redis port)

def get_file_name():
    """
    Generates a unique file name based on the current timestamp.
    """
    timestamp = datetime.datetime.now().strftime('%Y-%m-%d_%H-%M-%S')
    return f"dump_{timestamp}.txt"

def log_data(data, file):
    """
    Logs the data being forwarded to a file.
    """
    try:
        # Attempt to decode the data as UTF-8 and write it to the file
        file.write(data.decode('utf-8'))
    except UnicodeDecodeError:
        # If data can't be decoded to a UTF-8 string (binary data), write its hexadecimal representation
        file.write(data.hex())
    except Exception as e:
        # Catch all other exceptions and log them
        file.write(f"Error: {e}")
        raise e
    
    file.flush()

def handle_client(client_socket, client_address):
    """
    Handles the client connection.
    """
    server_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server_socket.connect((TARGET_HOST, TARGET_PORT))
    
    # Open a new file for dumping the data
    file_name = get_file_name()
    with open(file_name, 'w') as file:
        # Threads for handling data bi-directionally with file logging
        client_to_server = threading.Thread(target=forward, args=(client_socket, server_socket, file))
        server_to_client = threading.Thread(target=forward, args=(server_socket, client_socket, file))
        
        client_to_server.start()
        server_to_client.start()

        client_to_server.join()
        server_to_client.join()

    client_socket.close()
    server_socket.close()

def forward(source, destination, file):
    """
    Forwards data from the source socket to the destination socket, logging to a file.
    """
    while True:
        data = source.recv(4096)
        if len(data) == 0:
            # No more data to read
            break
        log_data(data, file)
        destination.send(data)

def start_proxy():
    """
    Starts the TCP proxy.
    """
    listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    listener.bind((LISTEN_HOST, LISTEN_PORT))
    listener.listen(5)  # Max backlog of connections
    
    print(f"[*] Listening as {LISTEN_HOST}:{LISTEN_PORT} ...")
    
    try:
        while True:
            client_socket, addr = listener.accept()
            print(f"[>] Received incoming connection from {addr[0]}:{addr[1]}")
            
            client_thread = threading.Thread(target=handle_client, args=(client_socket, f"{addr[0]}:{addr[1]}"))
            client_thread.start()
    except KeyboardInterrupt:
        print("\n[!] Keyboard interrupt received, shutting down.")
    finally:
        listener.close()
        print("[*] Proxy server shut down.")

if __name__ == '__main__':
    start_proxy()
