// Frozen types-mode special-member declaration responsibility regression.
struct C{~C()try{}catch(int n){int x=n;}};
