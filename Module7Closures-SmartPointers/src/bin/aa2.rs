struct DataRecord {
    id: u32,
    value: String,
}

fn extract(_source: &str) -> Vec<DataRecord> {
    vec![
        DataRecord { id: 1, value: String::from("data1") },
        DataRecord { id: 2, value: String::from("data2") },
    ]
}

fn transform_uppercase(record: DataRecord) -> DataRecord {
    DataRecord { id: record.id, value: record.value.to_uppercase() }
}

fn transform_add_prefix(record: DataRecord) -> DataRecord {
    DataRecord { id: record.id, value: format!("PREFIX_{}", record.value) }
}

fn load(records: Vec<DataRecord>) {
    for record in records {
        println!("Loaded: id={}, value={}", record.id, record.value);
    }
}

struct EtlPipeline {
    extract: fn(&str) -> Vec<DataRecord>,
    transform1: fn(DataRecord) -> DataRecord,
    transform2: fn(DataRecord) -> DataRecord,
    load: fn(Vec<DataRecord>),
}

impl EtlPipeline {
    fn new(
        extract: fn(&str) -> Vec<DataRecord>,
        transform1: fn(DataRecord) -> DataRecord,
        transform2: fn(DataRecord) -> DataRecord,
        load: fn(Vec<DataRecord>)
    ) -> Self {
        EtlPipeline { extract, transform1, transform2, load }
    }

    fn run(&self, source: &str) {
        let data = (self.extract)(source);
        let transformed_data: Vec<_> = data.into_iter()
            .map(self.transform1)
            .map(self.transform2)
            .collect();
        (self.load)(transformed_data);
    }
}

fn main() {
    let etl_pipeline = EtlPipeline::new(extract, transform_uppercase, transform_add_prefix, load);
    etl_pipeline.run("dummy_source");
}