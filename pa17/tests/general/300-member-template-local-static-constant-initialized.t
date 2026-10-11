// N3485 6.7p2, 3.6.2p2 (third bullet): a block-scope static aggregate of
// function addresses (in a member function template's specialization) is
// constant-initialized before its block is first entered; a jump past its
// declaration (allowed for a static, 6.7p3) still finds its value.
template<class T> struct manager { static int call() { return 10; } };
template<class T> struct invoker { static int call() { return 1; } };
struct table { int (*manager)(); int (*invoker)(); };
template<class R>
struct function_n
{
  template<class Functor>
  int assign_to(Functor, bool skip)
  {
    typedef manager<Functor> manager_type;
    typedef invoker<Functor> invoker_type;
    if (skip)
      goto after;
    {
      static const table stored = { &manager_type::call, &invoker_type::call };
after:
      return stored.manager != 0 ? stored.manager() + stored.invoker() : 0;
    }
  }
};
struct log_functor {};
int main()
{
  function_n<void> fn;
  return fn.assign_to(log_functor(), true) == 11 ? 0 : 1;
}
