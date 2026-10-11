// N3485 focus: 14.8.3 [temp.over]

template<class Graph>
struct graph_traits { typedef int vertex_descriptor; };

template<class T, class Tag, class Base>
struct named_params {};

struct graph {};
struct pred {};
struct dist {};
struct weight {};
struct index {};
struct compare {};
struct combine {};
struct visitor {};
struct named_choice { char data[1]; };
struct generic_choice { char data[2]; };

template<class VertexListGraph, class DijkstraVisitor, class PredecessorMap,
         class DistanceMap, class WeightMap, class IndexMap, class Compare,
         class Combine, class DistInf, class DistZero, class T, class Tag,
         class Base>
named_choice dijkstra_shortest_paths(
    const VertexListGraph&,
    typename graph_traits<VertexListGraph>::vertex_descriptor,
    PredecessorMap, DistanceMap, WeightMap, IndexMap, Compare, Combine,
    DistInf, DistZero, DijkstraVisitor,
    const named_params<T, Tag, Base>&)
{
  return named_choice();
}

template<class VertexListGraph, class DijkstraVisitor, class PredecessorMap,
         class DistanceMap, class WeightMap, class IndexMap, class Compare,
         class Combine, class DistInf, class DistZero, class ColorMap>
generic_choice dijkstra_shortest_paths(
    const VertexListGraph&,
    typename graph_traits<VertexListGraph>::vertex_descriptor,
    PredecessorMap, DistanceMap, WeightMap, IndexMap, Compare, Combine,
    DistInf, DistZero, DijkstraVisitor, ColorMap)
{
  return generic_choice();
}

int main()
{
  graph g;
  int s = 0;
  pred p;
  dist d;
  weight w;
  index i;
  compare c;
  combine plus;
  visitor v;
  named_params<int, int, int> params;
  named_choice result =
      dijkstra_shortest_paths(g, s, p, d, w, i, c, plus, 0, 0, v, params);
  return sizeof(result) == sizeof(named_choice) ? 0 : 1;
}
// VALIDATION: compile-pass
