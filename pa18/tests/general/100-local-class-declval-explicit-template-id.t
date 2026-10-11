template<class T>
T&& declval();

int main()
{
  struct F {
    char operator()(int);
  };

  typedef decltype(declval<F&>()(declval<int>())) result_type;
  result_type value = char(0);
  return static_cast<int>(value);
}
