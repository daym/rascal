unit consumer;
interface
uses holder;
function count(h : tholder) : longint;
implementation
function count(h : tholder) : longint;
begin
  count := h.blocks.fcount;
end;
end.
