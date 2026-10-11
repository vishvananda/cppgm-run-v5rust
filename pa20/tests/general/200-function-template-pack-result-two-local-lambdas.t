// A pack-expanded function-template result must retain the exact type of each
// local lambda after deduction inside a class-template specialization.
template<class... T>
struct pack {
};

template<class... T>
pack<T&&...> make_pack(T&&...)
{
  return {};
}

template<class T>
struct holder {
  void run()
  {
    make_pack([] {}, [] {});
  }
};

int main()
{
  holder<int>().run();
  return 0;
}
