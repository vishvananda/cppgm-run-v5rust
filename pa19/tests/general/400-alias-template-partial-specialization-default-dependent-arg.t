template<class EdgeListS, class VertexListS, class DirectedS,
         class VertexProperty = int, class EdgeProperty = int>
struct adjacency_list
{
};

template<class WeightType>
using adj_list_test_graph =
    adjacency_list<int, int, int, int, WeightType>;

template<class Graph, class Tag>
struct property_map
{
  typedef int type;
};

template<class Graph>
struct property_map<Graph, int>
{
  typedef long type;
};

template<class Graph>
struct vertex_index_installer
{
  typedef typename property_map<Graph, int>::type type;
};

template<class WeightType>
struct vertex_index_installer<adj_list_test_graph<WeightType> >
{
  typedef unsigned type;
};

template<class WeightType>
int run()
{
  typedef adj_list_test_graph<WeightType> Graph;
  return sizeof(typename vertex_index_installer<Graph>::type) == sizeof(unsigned) ? 0 : 1;
}

int main()
{
  return run<double>();
}
