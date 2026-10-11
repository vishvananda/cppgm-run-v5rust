int sink;

struct context
{
  template<class Value>
  class executor
  {
  public:
    executor() noexcept {}
    ~executor() noexcept { ++sink; }
    void execute() noexcept { ++sink; }

    template<class OtherValue>
    executor<OtherValue> require(const OtherValue &) const noexcept
    {
      return executor<OtherValue>();
    }
  };
};

int main()
{
  context::executor<int> value;
  long next_value = 0;
  value.require(next_value).execute();
  return sink != 2;
}
