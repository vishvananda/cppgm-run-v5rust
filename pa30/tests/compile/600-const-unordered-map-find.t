#include <string>
#include <unordered_map>
#include <type_traits>
#include <utility>

using M = std::unordered_map<std::string, int>;

// anti-cheat anchors: require <unordered_map>+<string> to really be compiled,
// and pin the const-find API surface this test was about.
static_assert(std::is_same<M::mapped_type, int>::value, "mapped_type");
static_assert(std::is_same<M::key_type, std::string>::value, "key_type");
static_assert(
  std::is_same<
    decltype(std::declval<const M&>().find(std::declval<const std::string&>())),
    M::const_iterator>::value,
  "const find returns const_iterator");
