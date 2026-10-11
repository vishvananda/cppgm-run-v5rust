#include <vector>

template <typename WeightType>
struct edge_info
{
  unsigned int x;
  unsigned int y;
  WeightType w;
};

template <typename WeightType = long>
void run_test_graph(unsigned int,
                    const std::vector<edge_info<WeightType>> &,
                    WeightType)
{
}

int main()
{
  run_test_graph(0, {}, 0);
  run_test_graph<int>(0, {}, 0);
  run_test_graph(1, {{0, 1, 2}}, 2);
  return 0;
}
