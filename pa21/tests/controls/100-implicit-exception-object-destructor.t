// The exception object's implicit destructor destroys its nontrivial member.
int destroyed;
struct Leaf { ~Leaf() { ++destroyed; } };
struct Failure { Leaf leaf; };
struct CatchOnly { Leaf leaf; };
void fail() { throw Failure{}; }
void visit() {}
int main()
{
    try { fail(); } catch (const Failure&) {}
    try { visit(); } catch (const CatchOnly&) {}
    return destroyed == 1 ? 0 : 1;
}
