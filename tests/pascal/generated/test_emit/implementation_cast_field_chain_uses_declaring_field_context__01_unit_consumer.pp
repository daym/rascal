unit consumer;
interface
uses baseunit;
function count(n : tnode) : longint;
implementation
uses holder;
function count(n : tnode) : longint;
begin
  count := tholder(n).blocks.count + tholder(n).blocks.getcount;
end;
end.
