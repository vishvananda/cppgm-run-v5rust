// The course evaluates a compound assignment RHS before the LHS address.
// Both operand calls must occur once, with the RHS converted before the read.
int sequence, value;
int right() { sequence = 1; return 3; }
int& left() { if (sequence != 1) sequence = 9; return value; }
int main()
{
  left() += right();
  return sequence != 1 || value != 3;
}
