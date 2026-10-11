#include <algorithm>
#include <utility>
#include <vector>
#include <cstddef>

void Order(std::vector<std::pair<std::pair<int, std::size_t>, std::size_t> >* ranked)
{
	std::sort(ranked->begin(), ranked->end());
}
