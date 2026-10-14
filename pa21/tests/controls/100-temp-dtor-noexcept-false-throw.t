// N3485 12.2p3, 15.4p14(-): a temporary's destructor declared noexcept(false)
// may throw at the end of the full-expression and the exception reaches the
// handler; expect exit 0
struct DT { ~DT() noexcept(false) { throw 1; } };
int main() { try { DT(); } catch (int) { return 0; } return 1; }
