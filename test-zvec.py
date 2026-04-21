
import zvec

existing_collection = zvec.open(
    path="/home/santhosh/work/wiki/topictrend/data/embedding_store/zvec/enwiki-categories",
    option=zvec.CollectionOption(read_only=True, enable_mmap=False),
)

