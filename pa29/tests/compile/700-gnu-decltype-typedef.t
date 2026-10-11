template<typename T>
struct iter_traits_box {
  typedef __decltype(*((T*)0)) ref_type;
};

iter_traits_box<int>::ref_type f(iter_traits_box<int>::ref_type x)
{
  return x;
}
