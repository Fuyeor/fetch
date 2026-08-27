// apps/engine/src/index/schema.rs

use tantivy::schema::{
    Field, IndexRecordOption, STORED, STRING, Schema, TextFieldIndexing, TextOptions,
};

pub(crate) type IndexResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Field handles kept together so query and document code cannot drift apart.
#[derive(Clone, Copy)]
pub(crate) struct SearchFields {
    pub(crate) document_id: Field,
    pub(crate) mapping_id: Field,
    pub(crate) source: Field,
    pub(crate) url: Field,
    pub(crate) title: Field,
    pub(crate) content: Field,
    pub(crate) updated_at: Field,
    pub(crate) image_url: Field,
    pub(crate) graph: Field,
    pub(crate) content_hash: Field,
    pub(crate) generation: Field,
}

/// Build stored fields and jieba-backed text fields for the stable document schema.
pub(crate) fn build_schema() -> (Schema, SearchFields) {
    let mut builder = Schema::builder();
    let text_options = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("jieba_search")
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        )
        .set_stored();
    let document_id = builder.add_text_field("document_id", STRING | STORED);
    let mapping_id = builder.add_text_field("mapping_id", STRING | STORED);
    let source = builder.add_text_field("source", STRING | STORED);
    let url = builder.add_text_field("url", STRING | STORED);
    let title = builder.add_text_field("title", text_options.clone());
    let content = builder.add_text_field("content", text_options);
    let updated_at = builder.add_text_field("updated_at", STRING | STORED);
    let image_url = builder.add_text_field("image_url", STRING | STORED);
    let graph = builder.add_text_field("graph", STRING | STORED);
    let content_hash = builder.add_text_field("content_hash", STRING | STORED);
    let generation = builder.add_text_field("generation", STRING | STORED);
    let schema = builder.build();
    (
        schema,
        SearchFields {
            document_id,
            mapping_id,
            source,
            url,
            title,
            content,
            updated_at,
            image_url,
            graph,
            content_hash,
            generation,
        },
    )
}

/// Resolve all field handles from an existing index and fail fast on incompatible schemas.
pub(crate) fn fields_from_schema(schema: &Schema) -> IndexResult<SearchFields> {
    Ok(SearchFields {
        document_id: schema.get_field("document_id")?,
        mapping_id: schema.get_field("mapping_id")?,
        source: schema.get_field("source")?,
        url: schema.get_field("url")?,
        title: schema.get_field("title")?,
        content: schema.get_field("content")?,
        updated_at: schema.get_field("updated_at")?,
        image_url: schema.get_field("image_url")?,
        graph: schema.get_field("graph")?,
        content_hash: schema.get_field("content_hash")?,
        generation: schema.get_field("generation")?,
    })
}
