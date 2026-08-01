unit u;
interface
type
  TOp = (op_a, op_b);
  TOps = set of TOp;
function matches(op : TOp) : boolean; overload;
function matches(ops : TOps) : boolean; overload;
procedure demo(op : TOp; ready : boolean);
implementation
function matches(op : TOp) : boolean;
begin
  matches := op = op_a;
end;
function matches(ops : TOps) : boolean;
begin
  matches := op_a in ops;
end;
procedure demo(op : TOp; ready : boolean);
begin
  if ready and matches(op) and matches([op]) then writeln('yes');
end;
end.
