from .serializer import Serializer
from pathlib import Path
import textwrap
import logging

logger = logging.getLogger(__name__)

class HeaderCopier(Serializer): 
    generic_headers_dir: Path
    shared_header_dir: Path
    
    def __init__(self, shared_header_dir: Path): 
        self.shared_header_dir = shared_header_dir
    
    def copy(self): 
        logger.info("Copying shared headers to %s", self.shared_header_dir)
        self._generate_result_header()
        logger.debug("Header copy complete")
        
    def _generate_result_header(self) -> None:
        logger.debug("Generating result.h")
        result_header: str = textwrap.dedent("""
            #pragma once
            #include <string>

            struct Result
            {
            private:
                Result(void *p, void (*d)(void *))
                {
                    this->type = OK;
                    this->payload = p;
                    this->deleter = d;
                }
                Result(std::string s)
                {
                    this->type = FAIL;
                    this->fail_rsn = std::move(s);
                }

            public:
                enum Type
                {
                    OK,
                    FAIL
                } type;
                void *payload;
                std::string fail_rsn;
                void (*deleter)(void *);
                Result(const Result &) = delete;
                Result &operator=(const Result &) = delete;
                Result(Result &&) = delete;
                Result &operator=(Result &&) = delete;
                static Result *ok(void *payload, void (*deleter)(void *)) { return new Result(payload, deleter); }
                static Result *fail(std::string fail_rsn) { return new Result(fail_rsn); }
            };
        """)
        
        self._dump_data(self.shared_header_dir / "result.h", result_header)      