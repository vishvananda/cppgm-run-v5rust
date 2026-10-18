// N3485 5.3.4p2 requires a parenthesized assignment-expression for
// allocated auto; this is not variable auto list deduction. expect reject
int main() { new auto{1}; }
