struct C{}; using X=decltype(C())::Unknown; using Y=decltype(C())::template Q<int>;
