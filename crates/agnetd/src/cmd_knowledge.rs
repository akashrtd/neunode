use anyhow::Result;

use crate::cli::{GlobalArgs, KnowledgeCommands};
use crate::output::OutputWriter;
use crate::state::AppState;

pub fn execute(cmd: &KnowledgeCommands, args: &GlobalArgs, state: &mut AppState) -> Result<()> {
    let writer = OutputWriter::new(args.output);
    match cmd {
        KnowledgeCommands::Query { subject, predicate, object, graph, limit } => query_knowledge(
            subject.as_deref(),
            predicate.as_deref(),
            object.as_deref(),
            graph.as_deref(),
            *limit,
            &writer,
            state,
        ),
        KnowledgeCommands::RegisterAgent { did, capabilities } => {
            register_agent(did, capabilities, &writer, state)
        }
        KnowledgeCommands::RegisterModel { did, cid, parent } => {
            register_model(did, cid, parent.as_deref(), &writer, state)
        }
        KnowledgeCommands::RegisterBounty { id, capabilities } => {
            register_bounty(id, capabilities, &writer, state)
        }
        KnowledgeCommands::JoinJob { did, job_id } => join_job(did, job_id, &writer, state),
        KnowledgeCommands::ListClasses => list_classes(&writer),
        KnowledgeCommands::ListPredicates => list_predicates(&writer),
    }
}

// ---------------------------------------------------------------------------
// Query
// ---------------------------------------------------------------------------

fn query_knowledge(
    subject: Option<&str>,
    predicate: Option<&str>,
    object: Option<&str>,
    graph: Option<&str>,
    limit: usize,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    if subject.is_none() && predicate.is_none() && object.is_none() && graph.is_none() {
        writer.write_status("No results found");
        return Ok(());
    }

    let db = state.db();
    let dict = neunode_knowledge::StringDictionary::new(db);
    let engine = neunode_knowledge::QueryEngine::new(db, &dict);

    let pattern = build_query_pattern(&dict, subject, predicate, object, graph)?;
    let results = engine.query(&pattern)?;

    if results.is_empty() {
        writer.write_status("No results found");
        return Ok(());
    }

    let limited: Vec<_> = results.into_iter().take(limit).collect();
    let headers = ["Subject", "Predicate", "Object", "Graph"];
    let rows: Vec<Vec<String>> = limited
        .iter()
        .map(|r| vec![r.subject.clone(), r.predicate.clone(), r.object.clone(), r.graph.clone()])
        .collect();
    writer.write_table(&headers, &rows);
    Ok(())
}

fn build_query_pattern(
    dict: &neunode_knowledge::StringDictionary,
    subject: Option<&str>,
    predicate: Option<&str>,
    object: Option<&str>,
    graph: Option<&str>,
) -> Result<neunode_knowledge::QueryPattern> {
    let mut strings = Vec::new();
    let mut refs = Vec::new();

    if let Some(s) = subject {
        strings.push(s.to_string());
    }
    if let Some(p) = predicate {
        strings.push(p.to_string());
    }
    if let Some(o) = object {
        strings.push(o.to_string());
    }
    if let Some(g) = graph {
        strings.push(g.to_string());
    }

    for s in &strings {
        refs.push(s.as_str());
    }

    let hashes = dict.batch_insert(&refs)?;
    let mut idx = 0usize;

    let sub_hash = subject.map(|_| {
        let h = hashes[idx];
        idx += 1;
        h
    });
    let pred_hash = predicate.map(|_| {
        let h = hashes[idx];
        idx += 1;
        h
    });
    let obj_hash = object.map(|_| {
        let h = hashes[idx];
        idx += 1;
        h
    });
    let graph_hash = graph.map(|_| {
        let h = hashes[idx];
        idx += 1;
        h
    });

    Ok(neunode_knowledge::QueryPattern {
        subject: sub_hash,
        predicate: pred_hash,
        object: obj_hash,
        graph: graph_hash,
    })
}

// ---------------------------------------------------------------------------
// RegisterAgent
// ---------------------------------------------------------------------------

fn register_agent(
    did: &str,
    capabilities: &str,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    if did.is_empty() {
        anyhow::bail!("agent DID cannot be empty");
    }

    let caps = parse_capabilities(capabilities);
    let db = state.db();
    let dict = neunode_knowledge::StringDictionary::new(db);
    let cap_refs: Vec<&str> = caps.iter().map(|s| s.as_str()).collect();
    let batch = neunode_knowledge::register_agent(&dict, did, &cap_refs)?;
    batch.apply(db)?;

    let out = serde_json::json!({
        "did": did,
        "capabilities": caps,
        "triples_inserted": batch.len(),
    });
    writer.write_json(&out);
    writer.write_status(&format!("Agent registered: {did}"));
    Ok(())
}

// ---------------------------------------------------------------------------
// RegisterModel
// ---------------------------------------------------------------------------

fn register_model(
    did: &str,
    cid: &str,
    parent: Option<&str>,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    if did.is_empty() {
        anyhow::bail!("owner DID cannot be empty");
    }
    if cid.is_empty() {
        anyhow::bail!("model CID cannot be empty");
    }

    let db = state.db();
    let dict = neunode_knowledge::StringDictionary::new(db);
    let batch = neunode_knowledge::register_model(&dict, did, cid, parent)?;
    batch.apply(db)?;

    let mut out = serde_json::json!({
        "owner": did,
        "cid": cid,
        "triples_inserted": batch.len(),
    });
    if let Some(p) = parent {
        out["parent"] = serde_json::json!(p);
    }
    writer.write_json(&out);
    writer.write_status(&format!("Model registered: {cid}"));
    Ok(())
}

// ---------------------------------------------------------------------------
// RegisterBounty
// ---------------------------------------------------------------------------

fn register_bounty(
    id: &str,
    capabilities: &str,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    if id.is_empty() {
        anyhow::bail!("bounty ID cannot be empty");
    }

    let caps = parse_capabilities(capabilities);
    let db = state.db();
    let dict = neunode_knowledge::StringDictionary::new(db);
    let cap_refs: Vec<&str> = caps.iter().map(|s| s.as_str()).collect();
    let batch = neunode_knowledge::register_bounty(&dict, id, &cap_refs)?;
    batch.apply(db)?;

    let out = serde_json::json!({
        "id": id,
        "required_capabilities": caps,
        "triples_inserted": batch.len(),
    });
    writer.write_json(&out);
    writer.write_status(&format!("Bounty registered: {id}"));
    Ok(())
}

// ---------------------------------------------------------------------------
// JoinJob
// ---------------------------------------------------------------------------

fn join_job(did: &str, job_id: &str, writer: &OutputWriter, state: &AppState) -> Result<()> {
    if did.is_empty() {
        anyhow::bail!("agent DID cannot be empty");
    }
    if job_id.is_empty() {
        anyhow::bail!("job ID cannot be empty");
    }

    let db = state.db();
    let dict = neunode_knowledge::StringDictionary::new(db);
    let batch = neunode_knowledge::join_training_job(&dict, did, job_id)?;
    batch.apply(db)?;

    let out = serde_json::json!({
        "agent": did,
        "job": job_id,
        "triples_inserted": batch.len(),
    });
    writer.write_json(&out);
    writer.write_status(&format!("Agent {did} joined job {job_id}"));
    Ok(())
}

// ---------------------------------------------------------------------------
// ListClasses
// ---------------------------------------------------------------------------

fn list_classes(writer: &OutputWriter) -> Result<()> {
    let classes = neunode_knowledge::all_classes();
    let headers = ["Class", "URI"];
    let rows: Vec<Vec<String>> =
        classes.iter().map(|c| vec![c.to_string(), neunode_knowledge::nn(c)]).collect();
    writer.write_table(&headers, &rows);
    Ok(())
}

// ---------------------------------------------------------------------------
// ListPredicates
// ---------------------------------------------------------------------------

fn list_predicates(writer: &OutputWriter) -> Result<()> {
    let predicates = neunode_knowledge::all_predicates();
    let headers = ["Predicate", "URI"];
    let rows: Vec<Vec<String>> =
        predicates.iter().map(|p| vec![p.to_string(), neunode_knowledge::nn(p)]).collect();
    writer.write_table(&headers, &rows);
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_capabilities(input: &str) -> Vec<String> {
    if input.is_empty() {
        return Vec::new();
    }
    input.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{human_writer, json_writer as test_writer, test_state};

    // ── parse_capabilities tests ──

    #[test]
    fn parse_capabilities_comma_separated() {
        let caps = parse_capabilities("NLP,Vision,RL");
        assert_eq!(caps, vec!["NLP", "Vision", "RL"]);
    }

    #[test]
    fn parse_capabilities_trims_whitespace() {
        let caps = parse_capabilities(" NLP , Vision , RL ");
        assert_eq!(caps, vec!["NLP", "Vision", "RL"]);
    }

    #[test]
    fn parse_capabilities_empty_string() {
        let caps = parse_capabilities("");
        assert!(caps.is_empty());
    }

    #[test]
    fn parse_capabilities_single() {
        let caps = parse_capabilities("Training");
        assert_eq!(caps, vec!["Training"]);
    }

    #[test]
    fn parse_capabilities_filters_empty_parts() {
        let caps = parse_capabilities("NLP,,Vision,");
        assert_eq!(caps, vec!["NLP", "Vision"]);
    }

    // ── register_agent tests ──

    #[test]
    fn register_agent_valid() {
        let state = test_state();
        let writer = test_writer();
        register_agent("did:neunode:test", "NLP,Vision", &writer, &state).unwrap();
    }

    #[test]
    fn register_agent_persists() {
        let state = test_state();
        let writer = test_writer();
        register_agent("did:neunode:persist", "Training", &writer, &state).unwrap();

        // Query back by subject
        let db = state.db();
        let dict = neunode_knowledge::StringDictionary::new(db);
        let engine = neunode_knowledge::QueryEngine::new(db, &dict);
        let pattern = neunode_knowledge::QueryPattern {
            subject: Some(neunode_knowledge::StringDictionary::hash("did:neunode:persist")),
            ..Default::default()
        };
        let results = engine.query(&pattern).unwrap();
        // type quad + capability quad = 2
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn register_agent_no_capabilities() {
        let state = test_state();
        let writer = test_writer();
        register_agent("did:neunode:nocap", "", &writer, &state).unwrap();

        let db = state.db();
        let dict = neunode_knowledge::StringDictionary::new(db);
        let engine = neunode_knowledge::QueryEngine::new(db, &dict);
        let pattern = neunode_knowledge::QueryPattern {
            subject: Some(neunode_knowledge::StringDictionary::hash("did:neunode:nocap")),
            ..Default::default()
        };
        let results = engine.query(&pattern).unwrap();
        // Only type quad
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn register_agent_empty_did_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(register_agent("", "NLP", &writer, &state).is_err());
    }

    #[test]
    fn register_agent_multiple_capabilities() {
        let state = test_state();
        let writer = test_writer();
        register_agent("did:neunode:multi", "A,B,C", &writer, &state).unwrap();

        let db = state.db();
        let dict = neunode_knowledge::StringDictionary::new(db);
        let engine = neunode_knowledge::QueryEngine::new(db, &dict);
        let pattern = neunode_knowledge::QueryPattern {
            subject: Some(neunode_knowledge::StringDictionary::hash("did:neunode:multi")),
            ..Default::default()
        };
        let results = engine.query(&pattern).unwrap();
        // type + 3 capabilities = 4
        assert_eq!(results.len(), 4);
    }

    // ── register_model tests ──

    #[test]
    fn register_model_valid() {
        let state = test_state();
        let writer = test_writer();
        register_model("did:neunode:dev", "ipfs://QmModel", None, &writer, &state).unwrap();
    }

    #[test]
    fn register_model_with_parent() {
        let state = test_state();
        let writer = test_writer();
        register_model(
            "did:neunode:dev",
            "ipfs://QmChild",
            Some("ipfs://QmParent"),
            &writer,
            &state,
        )
        .unwrap();

        // Should have type + ownsModel + dependsOn = 3 for the owner,
        // and type + dependsOn = 2 for the model CID
        let db = state.db();
        let dict = neunode_knowledge::StringDictionary::new(db);
        let engine = neunode_knowledge::QueryEngine::new(db, &dict);

        // Model CID should have type + dependsOn = 2 quads
        let pattern = neunode_knowledge::QueryPattern {
            subject: Some(neunode_knowledge::StringDictionary::hash("ipfs://QmChild")),
            ..Default::default()
        };
        let results = engine.query(&pattern).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn register_model_empty_did_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(register_model("", "ipfs://QmModel", None, &writer, &state).is_err());
    }

    #[test]
    fn register_model_empty_cid_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(register_model("did:neunode:dev", "", None, &writer, &state).is_err());
    }

    // ── register_bounty tests ──

    #[test]
    fn register_bounty_valid() {
        let state = test_state();
        let writer = test_writer();
        register_bounty("bounty:42", "NLP,RLHF", &writer, &state).unwrap();
    }

    #[test]
    fn register_bounty_persists() {
        let state = test_state();
        let writer = test_writer();
        register_bounty("bounty:99", "Training", &writer, &state).unwrap();

        let db = state.db();
        let dict = neunode_knowledge::StringDictionary::new(db);
        let engine = neunode_knowledge::QueryEngine::new(db, &dict);
        let pattern = neunode_knowledge::QueryPattern {
            subject: Some(neunode_knowledge::StringDictionary::hash("bounty:99")),
            ..Default::default()
        };
        let results = engine.query(&pattern).unwrap();
        // type + 1 capability = 2
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn register_bounty_empty_id_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(register_bounty("", "NLP", &writer, &state).is_err());
    }

    // ── join_job tests ──

    #[test]
    fn join_job_valid() {
        let state = test_state();
        let writer = test_writer();
        join_job("did:neunode:worker", "job:101", &writer, &state).unwrap();
    }

    #[test]
    fn join_job_persists() {
        let state = test_state();
        let writer = test_writer();
        join_job("did:neunode:worker2", "job:202", &writer, &state).unwrap();

        let db = state.db();
        let dict = neunode_knowledge::StringDictionary::new(db);
        let engine = neunode_knowledge::QueryEngine::new(db, &dict);
        let pattern = neunode_knowledge::QueryPattern {
            subject: Some(neunode_knowledge::StringDictionary::hash("did:neunode:worker2")),
            ..Default::default()
        };
        let results = engine.query(&pattern).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].predicate.contains("participatesIn"));
    }

    #[test]
    fn join_job_empty_did_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(join_job("", "job:1", &writer, &state).is_err());
    }

    #[test]
    fn join_job_empty_job_id_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(join_job("did:neunode:agent", "", &writer, &state).is_err());
    }

    // ── list_classes tests ──

    #[test]
    fn list_classes_ok() {
        let writer = human_writer();
        list_classes(&writer).unwrap();
    }

    #[test]
    fn list_classes_json() {
        let writer = test_writer();
        list_classes(&writer).unwrap();
    }

    // ── list_predicates tests ──

    #[test]
    fn list_predicates_ok() {
        let writer = human_writer();
        list_predicates(&writer).unwrap();
    }

    #[test]
    fn list_predicates_json() {
        let writer = test_writer();
        list_predicates(&writer).unwrap();
    }

    // ── query tests ──

    #[test]
    fn query_no_filters_returns_all() {
        let state = test_state();
        let writer = test_writer();
        // On a clean DB with no data, querying with no filters should succeed
        // and return empty results (match-everything pattern)
        assert!(query_knowledge(None, None, None, None, 20, &writer, &state).is_ok());
    }

    #[test]
    fn query_by_subject_after_register() {
        let state = test_state();
        let writer = test_writer();

        // Register agent first
        register_agent("did:neunode:queryable", "NLP", &writer, &state).unwrap();

        // Now query by subject
        let writer2 = human_writer();
        query_knowledge(Some("did:neunode:queryable"), None, None, None, 20, &writer2, &state)
            .unwrap();
    }

    #[test]
    fn query_empty_results() {
        let state = test_state();
        let writer = test_writer();
        // No data inserted — query by subject returns "No results found"
        query_knowledge(Some("did:neunode:nonexistent"), None, None, None, 20, &writer, &state)
            .unwrap();
    }

    #[test]
    fn query_with_limit() {
        let state = test_state();
        let writer = test_writer();
        register_agent("did:neunode:limited", "A,B,C", &writer, &state).unwrap();

        // Query with limit 1 — should succeed even though 4 triples exist
        let writer2 = test_writer();
        query_knowledge(Some("did:neunode:limited"), None, None, None, 1, &writer2, &state)
            .unwrap();
    }
}
