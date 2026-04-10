import { getSdkOperator, getSdkPlatform } from './apiClient'

export const createSdkPlatform = (accessToken: string) => {
  return getSdkPlatform(accessToken)
}

export const createSdkOperator = (accessToken: string, operatorId: string) => {
  return getSdkOperator(accessToken, operatorId)
}
