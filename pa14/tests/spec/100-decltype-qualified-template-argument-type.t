// A qualified decltype template argument must retain its type interpretation.

struct executor
{
};

struct source
{
  typedef executor executor_type;
};

template<class T>
struct type_holder
{
};

int main()
{
  source object;
  type_holder<decltype(object)::executor_type> type_result;
  (void)type_result;
  return 0;
}
