namespace forwarding_top_const {

typedef int (*fn)();

int target()
{
  return 1;
}

template<class T>
struct remove_reference
{
  typedef T type;
};

template<class T>
struct remove_reference<T &>
{
  typedef T type;
};

template<class T>
struct remove_reference<T &&>
{
  typedef T type;
};

template<class T>
T && forward(typename remove_reference<T>::type & value)
{
  return static_cast<T &&>(value);
}

struct storage {};
struct true_type {};
struct false_type {};

template<class Functor>
struct manager
{
  typedef true_type local_storage;

  template<class F>
  static int create(storage &, F && value, true_type)
  {
    return value();
  }

  template<class F>
  static int create(storage &, F && value, false_type)
  {
    return value();
  }

  template<class F>
  static int init(storage & out, F && value)
  {
    return create(out, forward<F>(value), local_storage());
  }

  static int clone(storage & out, Functor * slot)
  {
    return init(out, *const_cast<const Functor *>(slot));
  }
};

int run(fn * slot)
{
  storage out;
  *slot = target;
  return manager<fn>::clone(out, slot);
}

}
