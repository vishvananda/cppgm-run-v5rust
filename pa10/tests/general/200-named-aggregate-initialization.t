int target;
struct Record { unsigned char narrow; unsigned long wide; short signed_value; char text[4]; int& reference; volatile int status; };
int main() {
  Record object = {3, 4U, 5, "abc", {target}, 6};
  object.status;
  object.reference = 7;
  typedef int Pair[2];
  Pair{1, 2};
  return object.narrow + object.signed_value + object.text[1] + target;
}
