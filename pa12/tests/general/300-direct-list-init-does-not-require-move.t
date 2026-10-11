typedef unsigned long size_type;

class simple_barrier
{
  size_type count_;

public:
  explicit simple_barrier(size_type count) : count_(count) {}
  simple_barrier(simple_barrier &&) = delete;
  simple_barrier(const simple_barrier &) = delete;

  size_type count() const
  {
    return count_;
  }
};

int main()
{
  size_type threads_count = 4;
  simple_barrier barrier{threads_count};
  return barrier.count() == 4 ? 0 : 1;
}
