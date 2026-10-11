// VALIDATION: compile-pass
// N3485 focus: 14.8.2.4 [temp.deduct.partial], 14.8.3 [temp.over]

typedef unsigned long size_t;

struct graph {};
struct rng {};

template<class MutableGraph>
struct graph_traits
{
  typedef size_t vertices_size_type;
};

struct fixed_choice
{
  char data[1];
};

struct output_choice
{
  char data[2];
};

template<class MutableGraph, class RandNumGen>
fixed_choice generate_random_graph(
    MutableGraph &,
    typename graph_traits<MutableGraph>::vertices_size_type,
    typename graph_traits<MutableGraph>::vertices_size_type,
    RandNumGen &,
    bool = true,
    bool = false)
{
  return fixed_choice();
}

template<class MutableGraph, class RandNumGen,
         class VertexOutputIterator, class EdgeOutputIterator>
output_choice generate_random_graph(
    MutableGraph &,
    typename graph_traits<MutableGraph>::vertices_size_type,
    typename graph_traits<MutableGraph>::vertices_size_type,
    RandNumGen &,
    VertexOutputIterator,
    EdgeOutputIterator,
    bool = false)
{
  return output_choice();
}

int main()
{
  graph g;
  rng r;
  size_t n_vertices = 1;
  size_t n_edges = 2;
  fixed_choice result =
      generate_random_graph(g, n_vertices, n_edges, r, true, true);
  return sizeof(result) == sizeof(fixed_choice) ? 0 : 1;
}
