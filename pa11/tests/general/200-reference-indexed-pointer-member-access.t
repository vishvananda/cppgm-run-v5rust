// VALIDATION: compile-pass
// A scalar reference used as a built-in pointer subscript is converted to its
// referent value before the selected class object's member is accessed.

struct element
{
  int value;
};

int read(element * elements, int & subscript)
{
  return elements[subscript].value;
}

int read_unsigned(element * elements, unsigned & subscript)
{
  return elements[subscript].value;
}

int main()
{
  element values[3] = {{7}, {11}, {19}};
  int subscript = -1;
  unsigned index = 2;
  return read(values + 1, subscript) != 7 ||
    read_unsigned(values, index) != 19;
}
