// VALIDATION: compile-pass
// A fixed forwarding-reference function template must beat a catch-all
// trailing-pack fallback when both candidates have equal conversion ranks.

template<typename T>
T&& forward_like(T& value)
{
  return static_cast<T&&>(value);
}

struct identity
{
  template<typename T>
  T&& operator()(T&& value) const
  {
    return forward_like<T>(value);
  }
};

struct nat
{
};

template<typename... Args>
nat invoke(Args&&... args);

template<typename F, typename A>
auto invoke(F&& function, A&& argument)
    -> decltype(static_cast<F&&>(function)(static_cast<A&&>(argument)))
{
  return static_cast<F&&>(function)(static_cast<A&&>(argument));
}

long count_chars(char * first, char * last, const char& value, identity project)
{
  long result = 0;
  for(; first != last; ++first) {
    if(invoke(project, *first) == value) {
      ++result;
    }
  }
  return result;
}

int main()
{
  char data[3] = {'a', 'b', 'a'};
  return count_chars(data, data + 3, data[0], identity()) == 2 ? 0 : 1;
}
