template <typename T>
struct selected_arg { typedef void type; };

struct service {
  template <typename T, typename = void>
  struct wrapper;

  template <typename T>
  static int make(T& value)
  {
    return wrapper<T>(value).get();
  }
};

template <typename T>
struct service::wrapper<T, typename selected_arg<T>::type> {
  wrapper(T&) {}
  int get() const { return 7; }
};

struct object {};

int main()
{
  const object value = object();
  return service::make(value) == 7 ? 0 : 1;
}
