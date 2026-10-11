// Receiver evaluation remains part of the enclosing full expression.
typedef int I;
int calls;
I* pointer() { ++calls; throw 7; }
I& reference() { ++calls; throw 9; }
int main()
{
  int caught = 0;
  try { pointer()->~I(); } catch (int value) { caught += value; }
  try { reference().~I(); } catch (int value) { caught += value; }
  return calls == 2 && caught == 16 ? 0 : 1;
}
