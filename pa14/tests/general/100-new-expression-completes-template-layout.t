void *operator new(unsigned long);

struct Base {};
struct Node {};

template<class T>
struct Control : Base
{
  T *ptr;
  Control(T *p) noexcept : ptr(p) {}
};

template<class T>
struct Owner
{
  Base *control;

  Owner(T *p)
  {
    typedef Control<T> C;
    control = new C(p);
  }
};

int main()
{
  Node node;
  Owner<Node> owner(&node);
  return owner.control == 0 ? 1 : 0;
}
