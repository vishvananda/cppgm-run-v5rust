namespace {

template<class T>
struct Iter {
  Iter() {}

  template<class U>
  Iter(const Iter<U>&) {}
};

void f()
{
  struct A {};
  Iter<A> it;
  auto sink = [](Iter<const A>) {};
  sink(it);
}

}  // namespace

int main()
{
  f();
  return 0;
}
