struct NumberArray
{
  unsigned elems[4];
};

struct TextAndValue
{
  char text[3];
  int value;
};

int main()
{
  NumberArray numbers = { 7 };
  if(numbers.elems[0] != 7) {
    return 1;
  }
  if(numbers.elems[1] != 0) {
    return 2;
  }
  if(numbers.elems[3] != 0) {
    return 3;
  }

  TextAndValue text = { "hi", 11 };
  if(text.text[0] != 'h') {
    return 4;
  }
  if(text.text[1] != 'i') {
    return 5;
  }
  if(text.text[2] != 0) {
    return 6;
  }
  return text.value == 11 ? 0 : 7;
}
