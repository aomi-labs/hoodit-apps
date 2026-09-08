import { relayAgentRequest } from "../../../../lib/agent-relay";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";
export const maxDuration = 60;

export const GET = relayAgentRequest;
export const POST = relayAgentRequest;
export const PATCH = relayAgentRequest;
export const DELETE = relayAgentRequest;
