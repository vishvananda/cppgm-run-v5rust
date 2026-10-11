struct log_functor { };
struct init_functor { };
struct obj_tag { };

template<class T> struct manager;
template<> struct manager<log_functor> { static int call() { return 10; } };
template<> struct manager<init_functor> { static int call() { return 20; } };

template<class T> struct invoker;
template<> struct invoker<log_functor> { static int call() { return 1; } };
template<> struct invoker<init_functor> { static int call() { return 2; } };

template<class T> struct get_tag { typedef obj_tag type; };

template<class Tag> struct get_invoker;
template<> struct get_invoker<obj_tag>
{
  template<class T, class R, class... Args>
  struct apply_
  {
    typedef manager<T> manager_type;
    typedef invoker<T> invoker_type;
  };
};

struct table
{
  int (*manager)();
  int (*invoker)();
};

template<class R, class... Args>
struct function_n
{
  template<class Functor>
  int assign_to(Functor)
  {
    typedef typename get_tag<Functor>::type tag;
    typedef get_invoker<tag> get_invoker_type;
    typedef typename get_invoker_type::template apply_<Functor, R, Args...>::manager_type manager_type;
    typedef typename get_invoker_type::template apply_<Functor, R, Args...>::invoker_type invoker_type;
    static const table stored = { &manager_type::call, &invoker_type::call };
    return stored.manager() + stored.invoker();
  }
};

int main()
{
  function_n<void> fn;
  int a = fn.assign_to(log_functor());
  int b = fn.assign_to(init_functor());
  if(a != 11) return 1;
  if(b != 22) return 2;
  return 0;
}
