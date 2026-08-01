unit aoptcpub;
interface
uses cpubase;
type
  toper = record
    typ : longint;
    reg : tregister;
  end;
  poper = ^toper;
  tai_cpu_abstract = class
    oper : array[0..max_operands-1] of poper;
  end;
  taicpu = class(tai_cpu_abstract)
  end;
  tinstr = taicpu;
  pinstr = ^tinstr;
implementation
end.
