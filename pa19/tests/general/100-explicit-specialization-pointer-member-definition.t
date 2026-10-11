// Boost.Test print_log_value reduction:
// the out-of-class definition for an explicitly specialized class-template
// member must keep the pointer type argument structured.
template<typename T>
struct print_log_value
{
  void operator()(T);
};

template<>
struct print_log_value<char const*>
{
  void operator()(char const*);
};

void print_log_value<char const*>::operator()(char const*) {}

int main()
{
  return 0;
}
