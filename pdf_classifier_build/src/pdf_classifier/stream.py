import asyncio
import struct
import os 
import logging
import threading
import queue as thread_queue
from typing import AsyncIterator
from dataclasses import dataclass

logger = logging.getLogger(__name__)

@dataclass 
class ExtractionResult: 
    page: int
    class_id: str
    payload: str
    
    def __init__(self, page: int, class_id: str, payload: str): 
        self.page = page 
        self.class_id = class_id
        self.payload = payload

class Stream:
    _loop: asyncio.AbstractEventLoop
    _thread: threading.Thread
    _raw_queue: thread_queue.Queue
    port: int
    
    def __init__(self): 
        ready = threading.Event()
        self._loop = asyncio.new_event_loop()
        self._raw_queue = thread_queue.Queue()
     
        async def client_cb(reader: asyncio.StreamReader, _: asyncio.StreamWriter):
            logger.info("Creating callback.")
            while True:
                try: 
                    page_bytes = await reader.readexactly(4)
                    page = struct.unpack_from("<I", page_bytes)[0]
                    class_len_bytes = await reader.readexactly(4)
                    class_len = struct.unpack_from("<I", class_len_bytes)[0]
                    class_id = await reader.readexactly(class_len)
                    payload_len_bytes = await reader.readexactly(4)
                    payload_len = struct.unpack_from("<I", payload_len_bytes)[0]
                    payload = await reader.readexactly(payload_len)
                    self._raw_queue.put((page, class_id, payload))
                except: 
                    break 
            self._raw_queue.put(None)

        async def _start():
            server = await asyncio.start_server(client_cb, '127.0.0.1', 0)
            self.port = server.sockets[0].getsockname()[1]
            ready.set()
            async with server:
                await server.serve_forever()

        self._thread = threading.Thread(
            target=lambda: self._loop.run_until_complete(_start()),
            daemon=True,
        )
        
        self._thread.start()
        ready.wait()

        os.environ["CLASSIFIER_OUTPUT_PORT"] = str(self.port)
        logger.info(f"Spawned server on port {self.port}")
        
    async def stream_extraction_results(self) -> AsyncIterator[ExtractionResult]:
        loop = asyncio.get_event_loop()
        while True:
            item = await loop.run_in_executor(None, self._raw_queue.get)
            if item is None: 
                break 
            page, class_id, payload = item
            yield ExtractionResult(page, class_id, payload)

