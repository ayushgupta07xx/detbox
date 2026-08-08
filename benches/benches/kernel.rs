//! Kernel benchmarks: the real parse/serialize pair.
//!
//! These replaced the Phase-0 `roundtrip_identity` benchmarks when the parsers
//! landed (ADR-003). Benchmark names are a contract — `benches/baselines/*.tsv`
//! lists every one, and `xtask bench-compare` fails if a name appears in one
//! place and not the other — so this rename is a reviewed change, not a silent
//! one.
//!
//! Inputs are shaped like the corpus rather than like a microbenchmark: the YAML
//! carries comments, anchors, a block scalar and Go templating, because 41.2% of
//! real corpus files contain the last of those and a parser tuned on
//! template-free YAML would be tuned on the wrong thing.

// `criterion_group!`/`criterion_main!` expand to undocumented items. The
// workspace denies `missing_docs`; a macro's expansion is not ours to document.
#![allow(missing_docs)]

use core_formats::{Format, Json, Yaml};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

const YAML_SMALL: &[u8] = br#"# a small chart values file
replicaCount: 1
image:
  repository: nginx
  tag: "1.25"          # pinned deliberately
resources: {}
nodeSelector:
  kubernetes.io/os: linux
"#;

const YAML_REAL: &[u8] = br#"apiVersion: apps/v1
kind: Deployment
metadata:
  name: {{ include "chart.fullname" . }}
  labels:
    {{- include "chart.labels" . | nindent 4 }}
spec:
  replicas: {{ .Values.replicaCount }}
  template:
    spec:
      containers:
        - name: {{ .Chart.Name }}
          image: "{{ .Values.image.repository }}:{{ .Values.image.tag }}"
          command:
            - /bin/sh
            - -c
            - |
              set -eu
              echo "starting"        # a block scalar
              exec /usr/bin/app --config /etc/app.yaml
          env: &env
            - name: LOG_LEVEL
              value: info
          envFrom: *env
"#;

const JSON_REAL: &[u8] =
    br#"{"name":"example","version":"1.2.3","dependencies":{"a":"^1.0.0","b":"~2.3.4"},
"scripts":{"build":"tsc -p .","test":"jest --ci"},"keywords":["one","two","three"],
"numbers":[0,-0,1.0,1e5,1E+5,-1.5e-10,123456789012345678901234567890],
"nested":{"deep":{"deeper":{"deepest":[{"k":null},{"k":true},{"k":false}]}}}}"#;

fn repeat(input: &[u8], times: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() * times);
    for _ in 0..times {
        out.extend_from_slice(input);
    }
    out
}

fn parse_benches(c: &mut Criterion) {
    let yaml_64k = repeat(YAML_REAL, 100);
    let json_64k = repeat(JSON_REAL, 120);

    c.bench_function("yaml_parse_small", |b| {
        b.iter(|| Yaml.parse(black_box(YAML_SMALL)).map(|c| c.serialize()));
    });
    c.bench_function("yaml_parse_real", |b| {
        b.iter(|| Yaml.parse(black_box(YAML_REAL)).map(|c| c.serialize()));
    });
    c.bench_function("yaml_parse_64kib", |b| {
        b.iter(|| Yaml.parse(black_box(&yaml_64k)).map(|c| c.serialize()));
    });
    c.bench_function("json_parse_real", |b| {
        b.iter(|| Json.parse(black_box(JSON_REAL)).map(|c| c.serialize()));
    });
    c.bench_function("json_parse_64kib", |b| {
        b.iter(|| Json.parse(black_box(&json_64k)).map(|c| c.serialize()));
    });
}

criterion_group!(benches, parse_benches);
criterion_main!(benches);
