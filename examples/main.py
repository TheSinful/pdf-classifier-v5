from pathlib import Path
from pdf_classifier import Builder, ObjectFactory, BlankAfterClassOverride, MultiPageHierarchyBreakOverride, Stream, ExtractionResult
from dataclasses import dataclass
from typing import Optional
import logging
import json
import asyncio
import threading 

logging.basicConfig(
    level=logging.DEBUG,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
)

async def main(): 
    factory = ObjectFactory("test.hpp")

    factory.new().name("chapter").header("chapter.hpp").classify("classify_chapter").extract("extract_chapter").organizational().build()
    factory.new().name("subchapter").header("subchapter.hpp").classify("classify_subchapter").extract("extract_subchapter").child_of("chapter").organizational().build()
    factory.new().name("diagram").header("diagram.hpp").classify("classify_diagram").extract("extract_diagram").child_of("subchapter").pair_to("datatable", 1).build()
    factory.new().name("datatable").header("table.hpp").classify("classify_datatable").extract("extract_datatable").child_of("subchapter").pair_to("diagram", 2).build()

    examples_root = Path(__file__).parent

    build = Builder(examples_root / "build", factory, examples_root / "CMakeLists.txt", )
    build.override(BlankAfterClassOverride("chapter"))
    build.override(MultiPageHierarchyBreakOverride("chapter", True, "subchapter", ["diagram", "datatable"]))

    stream: Stream = build.build(skip_user_build=True)

    test_doc_path = Path(__file__).parent.parent / "data" / "large_test_doc.pdf"
    start_page = 44
    end_page = 1277
    num_threads = 4; 
    
    await build.spawn_classifier(start_page, end_page, num_threads, test_doc_path, False)
    
    seen = {}  
    pages: dict[int, str] = {}
    async for result in stream.stream_extraction_results():
        page = int(result.page)
        class_str = str(result.class_id, 'utf-8')

        if page in seen:
            del seen[page]
            seen[page] = class_str
            for p, c in seen.items():
                pages[p] = c
        else:
            seen[page] = class_str
            pages[page] = class_str 
    

    sorted_list: dict[int, str] = dict(sorted(pages.items()))
    with open("./data.txt", 'w') as f:
        for page_num, class_name in sorted_list.items(): 
            f.write(f"{page_num} {class_name}\n")
class _Object: 
    ...

@dataclass 
class TableData: 
    annotation: str
    count: int 
    dmc_army: str
    is_included_in_diagram: bool
    key: int 
    name: str 
    nato_stock_num: str
    part_num: str

@dataclass
class DataBlock(_Object): 
    name: str 
    table_data: TableData
    
    async def extract_diagram_image(self, page: int, path: Path) -> None:
        ...
        
    async def extract_datatable_image(self, page: int, path: Path) -> None:
        ...

@dataclass 
class SubChapter(_Object): 
    blocks: list[DataBlock]
    num: str
    
@dataclass 
class ExpectedSubChapter: 
    path: str
    sub_chapter_num: str
    
@dataclass
class Chapter(_Object): 
    data_blocks: list[DataBlock]
    sub_chapters: list[SubChapter]
    expected_sub_chapters: list[ExpectedSubChapter]
    num: str

    

class Serializer: 
    results: list[ExtractionResult]
    
    def __init__(self):
        self.results = []
        pass
    
    def push_result(self, result: ExtractionResult) -> None: 
        self.results.append(result)
    
    def compile_results(self) -> None: 
        pass
    
    def _construct_chapters(self) -> list[Chapter]:
        orphaned_children: list[_Object] = []
        current_chapter: Optional[Chapter] = Chapter([], [], [], "")
        current_sub_chapter: Optional[SubChapter] = None
        start_of_datablock: Optional[DataBlock] = None
        
        for result in self.results:
            extracted_data = json.loads(result.payload)            
            
            match result.class_id: 
                case "chapter":
                    expected_sub_chapters: list[ExpectedSubChapter] = []
                    for sub in extracted_data["sub_chapters"]: 
                        expected_sub_chapters.append(ExpectedSubChapter(sub["path"], sub["sub_chapter_num"]))
                    
                    current_chapter = Chapter([], [], expected_sub_chapters, extracted_data["chapter_num"])
                case "subchapter":
                    if not current_chapter: 
                        raise RuntimeError("Found a SubChapter that doesn't exist under a Chapter")
                    sub_chapter_num = extracted_data["subchapter_num"]
                    if sub_chapter_num not in current_chapter.expected_sub_chapters: 
                        raise RuntimeError(f"SubChapter {sub_chapter_num} is not apart of the expected subchapter numbers of chapter {current_chapter.num}")
                    
                    current_chapter.sub_chapters.append(SubChapter([], sub_chapter_num))

        return []
            
                        
        
    
    

if __name__ == "__main__": 
    asyncio.run(main())