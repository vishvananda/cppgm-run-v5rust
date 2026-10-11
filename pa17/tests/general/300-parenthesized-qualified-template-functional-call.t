namespace outer {
namespace inner {

struct empty_work {
};

template<class T>
struct dispatch {
  explicit dispatch(const T &)
  {
  }

  template<class Function, class Work>
  void operator()(Function &, Work) const
  {
  }
};

}
}

void handler()
{
}

int run()
{
  typedef int executor_type;
  executor_type ex = 1;
  (outer::inner::dispatch<executor_type>(ex))(handler, outer::inner::empty_work());
  return 0;
}
