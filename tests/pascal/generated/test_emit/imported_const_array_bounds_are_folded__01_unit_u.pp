unit u;
interface
uses cpubase;
type
  poper = ^longint;
  tinst = record
    oper : array[0..max_operands-1] of poper;
  end;
implementation
end.
