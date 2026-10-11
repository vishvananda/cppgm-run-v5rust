// VALIDATION: compile-pass
// N3485 focus: 14.8.2.1 [temp.deduct.call]

template<class T>
char (&array_reference_probe(T))[1];

int main()
{
  int value = sizeof(array_reference_probe(0));
  return value == 1 ? 0 : 1;
}
