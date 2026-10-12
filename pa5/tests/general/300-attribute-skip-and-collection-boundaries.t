// Pointer, base and trailing parameter attributes use the oracle's balanced skip path.
struct C {};
struct D : [[noreturn(ignored)]] C {};
int * [[noreturn(ignored)]] const [[gnu::section(1)]] pointer;
void f(int [[vendor::unused]] = 1);
[[course_attribute((x,y), {z}, [w])]];
[[,vendor::ignored((), {}, []),]];
__attribute__((unknown((a),{b},[c]))) int x;
