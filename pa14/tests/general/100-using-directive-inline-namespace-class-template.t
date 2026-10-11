namespace helper_inline_ns {
inline namespace v1 {
template<class T>
struct vec {
  explicit vec(T init) : value(init) {}
  T value;
};
}  // namespace v1
}  // namespace helper_inline_ns

namespace using_namespace_vector_link {
int total(const helper_inline_ns::vec<int> & value);
}  // namespace using_namespace_vector_link

using namespace helper_inline_ns;

namespace using_namespace_vector_link {
int total(const vec<int> & value)
{
  return value.value;
}
}  // namespace using_namespace_vector_link

int main()
{
  helper_inline_ns::vec<int> value(6);
  return using_namespace_vector_link::total(value) == 6 ? 0 : 1;
}
