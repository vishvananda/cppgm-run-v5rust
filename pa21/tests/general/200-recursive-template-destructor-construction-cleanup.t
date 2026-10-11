// A generated cleanup dependency may demand this class-template destructor
// only after the recursively containing type is complete.  The emitted
// destructor and its unwind cleanup must retain the same concrete owner.

int destroyed;

template<class T>
struct owning
{
  T* pointer;

  owning() : pointer(0)
  {
  }

  ~owning()
  {
    (void)sizeof(T);
    ++destroyed;
    pointer = 0;
  }
};

template<class T>
struct box
{
  owning<T> value;

  box() : value()
  {
  }
};

struct outer
{
  box<outer> value;
};

int main()
{
  {
    outer value;
    if(value.value.value.pointer != 0) {
      return 1;
    }
  }
  return destroyed == 1 ? 0 : 1;
}
