namespace abi_case {
template<class T> struct traits { };
template<class T> struct alloc { };
inline namespace v2 {
template<class C, class Traits, class Alloc>
struct buffer {
  int value;
};
}  // namespace v2
template<class C, class Traits>
struct stream {
  int value;
};
template<class C, class Traits, class Alloc>
stream<C, Traits>& fetch(stream<C, Traits>& in, buffer<C, Traits, Alloc>& out)
{
  (void)out;
  return in;
}
}  // namespace abi_case

int main()
{
  abi_case::stream<char, abi_case::traits<char> > in;
  abi_case::buffer<char, abi_case::traits<char>, abi_case::alloc<char> > out;
  out.value = 0;
  return &abi_case::fetch(in, out) == &in ? out.value : 1;
}
