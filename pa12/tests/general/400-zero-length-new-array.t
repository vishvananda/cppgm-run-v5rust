// A zero bound in new[] is allowed; no element is constructed or destroyed.
int constructed;
int destroyed;
struct Element
{
  Element() { ++constructed; }
  ~Element() { ++destroyed; }
};
int main()
{
  Element* values = new Element[0]{};
  delete[] values;
  return constructed == 0 && destroyed == 0 ? 0 : 1;
}
