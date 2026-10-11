// VALIDATION: compile-pass
// Boost.Interprocess reduction: a function-template argument can be a
// constructor call whose constructor parameter resolves &function_template
// through the target function-pointer type.

namespace qft_ctor_arg
{
typedef int handle;

template<class F>
int launch(handle & h, F f)
{
  (void)h;
  f();
  return 0;
}

template<class P>
struct adapter
{
  adapter(void (*func)(void *, P &), void * arg, P & object)
      : func_(func), arg_(arg), object_(object)
  {
  }

  void operator()() const
  {
    func_(arg_, object_);
  }

  void (*func_)(void *, P &);
  void * arg_;
  P & object_;
};

template<class M>
void lock_and_sleep(void * arg, M & mtx)
{
  (void)arg;
  (void)mtx;
}

}

struct mutex_type {};

int main()
{
  mutex_type mtx;
  qft_ctor_arg::handle h = 0;
  return qft_ctor_arg::launch(
      h,
      qft_ctor_arg::adapter<mutex_type>(
          &qft_ctor_arg::lock_and_sleep, 0, mtx));
}
