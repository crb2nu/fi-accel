from fi_accel import summarize_accel_observations


def test_summarize_accel_observations_ranks_hot_paths():
    data = b"""
{"service":"gateway","operation":"parse_jsonrpc","duration_ms":10.0,"input_bytes":100,"fallback":false}
{"service":"gateway","operation":"parse_jsonrpc","duration_ms":20.0,"input_bytes":200,"fallback":true}
{"service":"gateway","operation":"policy_check","duration_ms":2.0,"input_bytes":10,"fallback":false}
"""

    summaries = summarize_accel_observations(data)

    assert summaries[0]["operation"] == "parse_jsonrpc"
    assert summaries[0]["service"] == "gateway"
    assert summaries[0]["sample_count"] == 2
    assert summaries[0]["total_duration_ms"] == 30.0
    assert summaries[0]["avg_duration_ms"] == 15.0
    assert summaries[0]["p95_duration_ms"] == 20.0
    assert summaries[0]["max_duration_ms"] == 20.0
    assert summaries[0]["total_input_bytes"] == 300
    assert summaries[0]["avg_input_bytes"] == 150.0
    assert summaries[0]["fallback_count"] == 1
    assert summaries[0]["fallback_rate"] == 0.5


def test_summarize_accel_observations_supports_custom_nested_fields():
    data = b"""
{"svc":"news","name":"chunk","metrics":{"ms":"4","bytes":"1000","fallback":"yes"}}
{"svc":"news","name":"chunk","metrics":{"ms":6,"bytes":2000,"fallback":false}}
{"svc":"news","name":"embed","metrics":{"ms":100,"bytes":3000,"fallback":false}}
"""

    summaries = summarize_accel_observations(
        data,
        operation_field="name",
        service_field="svc",
        duration_ms_field="metrics.ms",
        input_bytes_field="metrics.bytes",
        fallback_field="metrics.fallback",
        limit=2,
    )

    assert len(summaries) == 1
    assert summaries[0]["operation"] == "chunk"
    assert summaries[0]["sample_count"] == 2
    assert summaries[0]["total_input_bytes"] == 3000
    assert summaries[0]["fallback_count"] == 1
