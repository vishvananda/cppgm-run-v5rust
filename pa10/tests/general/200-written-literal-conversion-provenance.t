int global;
long return_cast() { return (int)3; }
long return_plus() { return +3; }
long return_comma() { return (0, 3); }
int read(const long& value) { return value; }
struct Record { long value; };
int main() {
  long cast = (int)3;
  long functional = short(4);
  long literal = (5);
  Record object = {(int)6};
  int values[2] = {7, 8};
  int* pointer = values + (short)1;
  return return_cast() + return_plus() + return_comma() + cast +
    functional + literal + object.value + read((int)9) + *pointer +
    ((short)10 + global);
}
