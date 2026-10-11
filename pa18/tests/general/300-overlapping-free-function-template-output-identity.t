struct directed {};
struct distributed {};

template<class Direction, class Selector>
struct graph {};

template<class Direction, class Selector>
int vertex(graph<Direction, Selector> const&) {
  return 1;
}

template<class Selector>
int vertex(graph<directed, Selector> const&) {
  return 2;
}

int main() {
  graph<directed, distributed> instance;
  return vertex(instance) == 2 ? 0 : 1;
}
