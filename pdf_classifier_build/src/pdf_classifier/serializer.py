from pathlib import Path
import logging

logger = logging.getLogger(__name__)

class Serializer:     
    def _fmt_payload(self, payload: list[str], prefix: str = "\n") -> str: 
        return prefix.join(payload) 
    
    def _dump_data(self, into: Path, data: str) -> None: 
        logger.debug("Writing %d bytes to %s", len(data), into)
        with open(into, "w") as f: 
            written = f.write(data)
            
            if written <= 0: 
                raise RuntimeError("Failed to write data to file.")
        logger.debug("Successfully wrote %s", into)