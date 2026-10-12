namespace N{typedef int word;}
struct C{friend int N::x=sizeof(word), N::y=sizeof(word);};
