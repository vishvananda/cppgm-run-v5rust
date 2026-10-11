// N3485 focus: 14.8.2 [temp.deduct], substituted function result types

template<class Iterator, class IndexMap, class T, class R>
struct iterator_property_map {};

template<class T>
struct vector {
  typedef T * iterator;
};

template<class T>
struct identity_map {};

struct directedS {};
struct no_property {};
struct vertex_all_t {};
struct vertex_index_t {};

template<class Derived, class Property, class Descriptor, class IndexMap>
struct indexed_vertex_properties {
  typedef iterator_property_map<typename vector<Property>::iterator,
                                IndexMap, Property, Property&> vertex_map_type;

  vertex_map_type get_vertex_bundle(const IndexMap& index = IndexMap())
  {
    (void)index;
    return vertex_map_type();
  }
};

template<class Directed, class VertexProperty, class EdgeProperty,
         class GraphProperty, class Vertex = unsigned long,
         class EdgeIndex = Vertex>
struct graph;

template<class VertexProperty, class EdgeProperty, class GraphProperty,
         class Vertex, class EdgeIndex>
struct graph<directedS, VertexProperty, EdgeProperty,
             GraphProperty, Vertex, EdgeIndex>
  : indexed_vertex_properties<
        graph<directedS, VertexProperty, EdgeProperty,
              GraphProperty, Vertex, EdgeIndex>,
        VertexProperty, Vertex, identity_map<Vertex> > {
  typedef indexed_vertex_properties<
      graph<directedS, VertexProperty, EdgeProperty,
            GraphProperty, Vertex, EdgeIndex>,
      VertexProperty, Vertex, identity_map<Vertex> > inherited_vertex_properties;
};

template<class Graph, class Tag>
struct property_map;

template<class Directed, class VertexProperty, class EdgeProperty,
         class GraphProperty, class Vertex, class EdgeIndex>
struct property_map<
    graph<Directed, VertexProperty, EdgeProperty,
          GraphProperty, Vertex, EdgeIndex>,
    vertex_all_t> {
  typedef typename graph<Directed, VertexProperty, EdgeProperty,
                         GraphProperty, Vertex, EdgeIndex>
      ::inherited_vertex_properties::vertex_map_type type;
};

template<class Directed, class VertexProperty, class EdgeProperty,
         class GraphProperty, class Vertex, class EdgeIndex>
identity_map<Vertex> get(
    vertex_index_t,
    graph<Directed, VertexProperty, EdgeProperty,
          GraphProperty, Vertex, EdgeIndex>&)
{
  return identity_map<Vertex>();
}

template<class Directed, class VertexProperty, class EdgeProperty,
         class GraphProperty, class Vertex, class EdgeIndex>
typename property_map<
    graph<Directed, VertexProperty, EdgeProperty,
          GraphProperty, Vertex, EdgeIndex>,
    vertex_all_t>::type
get(vertex_all_t,
    graph<Directed, VertexProperty, EdgeProperty,
          GraphProperty, Vertex, EdgeIndex>& g)
{
  return g.get_vertex_bundle(get(vertex_index_t(), g));
}

struct Vertex { double centrality; };
struct Edge {};

int main()
{
  typedef graph<directedS, Vertex, Edge, no_property,
                unsigned long, unsigned long> graph_type;
  typedef iterator_property_map<Vertex *, identity_map<unsigned long>,
                                Vertex, Vertex&> expected_map;
  graph_type g;
  expected_map m = get(vertex_all_t(), g);
  (void)m;
  return 0;
}
// VALIDATION: compile-pass
