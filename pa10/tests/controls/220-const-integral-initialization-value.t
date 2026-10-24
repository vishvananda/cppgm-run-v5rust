// N3485 5.19p2 and 7.1.6.1: a const integral local stores its evaluated
// initializer; quotient-of-sizeof operands must not be lowered a second time.
// expect exit 0
int main() {
    int values[12];
    const unsigned count = sizeof values / sizeof values[0];
    const int arithmetic = (3 + 5) * 7 - 2;
    return count == 12 && arithmetic == 54 ? 0 : 1;
}
