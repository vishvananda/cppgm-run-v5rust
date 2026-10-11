// VALIDATION: compile-pass
// A class-template member body may call an unqualified function whose only
// argument has a dependent parameter type. Ordinary lookup is empty at the
// definition point, but ADL can find the function at instantiation.

namespace lib {

template<class G>
struct graph_traits
{
  typedef int vertices_size_type;
};

template<class G>
struct vertex_list_graph
{
  typedef typename graph_traits<G>::vertices_size_type vertices_size_type;

  void const_constraints(const G& cg)
  {
    count = num_vertices(cg);
  }

  vertices_size_type count;
};

}  // namespace lib

namespace user {

struct graph {};

int num_vertices(const graph&)
{
  return 7;
}

}  // namespace user

int main()
{
  lib::vertex_list_graph<user::graph> check;
  user::graph g;
  check.const_constraints(g);
  return check.count == 7 ? 0 : 1;
}
