// N3485 5.2.7p9, 5.2.8p2, 12.2p3 and 15.2p1: RTTI failure
// destroys evaluated temporaries and lexical objects; expect exit 0
int live,calls;
struct G {G(){++live;} ~G(){--live;}};
struct B {virtual ~B(){++calls;}};
struct D:B {};
void probe(){G g;B b;dynamic_cast<D&>(b);}
int main(){try{probe();return 1;}catch(...){return live || calls!=1 ? 2:0;}}
