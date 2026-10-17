// N3485 5.19p2: preserve rejection of a pseudo-destructor call during constant evaluation.
// expect reject
typedef int Integer;
constexpr int evaluate() { Integer value = 3; value.~Integer(); return 0; }
constexpr int result = evaluate();
