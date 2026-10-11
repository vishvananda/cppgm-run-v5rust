#include <functional>
#include <typeinfo>
#include <unordered_map>

struct type_info_hash
{
  std::size_t operator()(const std::type_info&) const noexcept { return 0; }
};

struct type_info_equal
{
  bool operator()(const std::type_info& x, const std::type_info& y) const noexcept
  {
    return x == y;
  }
};

using map_type = std::unordered_map<
    std::reference_wrapper<const std::type_info>, int,
    type_info_hash, type_info_equal>;

map_type values;
