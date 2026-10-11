// VALIDATION: compile-pass
// A dependent member return type retains its rvalue-reference category.

struct payload
{
  int value;
};

template<class T>
struct move_view
{
  typedef T&& reference;

  T *pointer;

  reference get() const
  {
    return static_cast<reference>(*pointer);
  }
};

int main()
{
  payload value;
  value.value = 19;

  move_view<payload> view;
  view.pointer = &value;
  return view.get().value == 19 ? 0 : 1;
}
