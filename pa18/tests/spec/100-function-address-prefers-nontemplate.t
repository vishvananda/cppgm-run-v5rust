// N3485 focus: 13.4 [over.over], 14.8.2.2 [temp.deduct.funcaddr]
// When a target function type identifies both an ordinary function and a
// function-template specialization, overload resolution selects the ordinary
// function.

int choose(int)
{
  return 0;
}

template<class T>
int choose(T)
{
  return 1;
}

typedef int (*choice)(int);

choice selected = choose;

template<class T>
T choose_void(T const&)
{
  return T();
}

void choose_void()
{
}

typedef void (*void_choice)();

// Substituting void into the template candidate would form const void&.  That
// invalid candidate is discarded before the ordinary overload is selected.
void_choice selected_void = choose_void;

int main()
{
  selected_void();
  return selected(1);
}
