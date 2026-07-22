"""Task 8 failing-first smoke test for the `linalg` bindings (spec §9.1 conformance harness).

Not one of the 51 ported oracle files — a hand-authored Task-8 gate proving
`incidence_matrix` (dense return) round-trips through the Rust core before the fuller
XGI `tests/linalg/test_matrix.py` module is ported in Task 17.
"""


def test_incidence_matrix_dense_shape():
    import numpy as np
    import hypergraph_rs as xgi

    H = xgi.Hypergraph([{1, 2, 3}, {4}, {5, 6}, {6, 7, 8}])  # edgelist1: 8 nodes, 4 edges
    I = xgi.incidence_matrix(H, sparse=False)
    assert isinstance(I, np.ndarray) and I.shape == (8, 4)
